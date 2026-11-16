-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.ClosureTyping
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! # The result's fact at an invocation the proof does not see

An invocation of a function value the proof does not see concludes the
result's own fact from the value's `ValidResultsAt`
(`wp_closureMeaning_unseen_at`).  Under the hypotheses of a run of such a
value and of its result lying outside its type's predicate, that fact and
the conclusion are both refuted: under those hypotheses the result's fact is
not a consequence of the invocation alone.  The result row here is a type
parameter, whose predicate is the family's own (`Carriers.holds`,
`NTy.holds_param`) and not the `True` of an invariant-free row, which is the
domain the rule is stated for (`ScalarAt`).

The statements take the run as a hypothesis. They do not claim that a
family resolving the parameter to a scalar admits such a run: that is a
question about a model, and the rule's premises are not shown inhabited
here.
-/

namespace VerexIR.Tests.UnseenResultValidity

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
  {executable : Validation.ExecutableUnit unit} [Skolems unit]

/-- A result row stating one type parameter: the family's predicate of a
value of it is its own field, which no reduction makes `True`. -/
abbrev shape : ResultShape := .one (.param 0)

/-- The row of the invocation's one scalar argument. -/
abbrev row : NRow := .cons (.int 64 false) .nil

section
variable {shared : Sharing}
  {function : (NTy.function row shared shape.row).carrier}
  {args : HList row} {result : shape.carrier} {initial final : Memory unit}

/-- A run returning a value outside the result's predicate refutes the
validity of the value's results. -/
theorem invalid_run_refutes_validResults
    (validArgs : HList.Holds row args)
    (ran : EnsuresOf (closureMeaning executable) row shared shape function args result
      initial final)
    (invalid : ¬shape.Holds result) :
    ¬ValidResultsAt executable row shared shape function :=
  fun valid => invalid (valid args result initial final validArgs ran)

/-- The same run refutes the conclusion the rule draws of every result:
under the hypotheses of the run and of its result's invalidity, the result's
fact is not a consequence of the invocation alone. -/
theorem invalid_run_refutes_result_fact
    (ran : EnsuresOf (closureMeaning executable) row shared shape function args result
      initial final)
    (invalid : ¬shape.Holds result) :
    ¬wp (closureMeaning executable row shared shape function args)
      (fun result _ => shape.Holds result) (fun _ => True) initial :=
  fun holds => invalid (holds.1 result final ran)

/-- With the fact assumed, every result of the invocation holds its own:
the rule for an invocation the proof does not see, at the arguments' own
facts. -/
theorem validResults_gives_result_fact
    (natives : NativesTyped executable) (agrees : RunsAgree executable)
    (agree : TypesAgree unit) (representable : OwnersRepresentable unit)
    (invocable : row.invocable = true) (shapeInvocable : shape.row.invocable = true)
    (free : row.refFree = true) (shapeFree : shape.row.refFree = true)
    (parametersScalar : ScalarAt unit row) (resultsScalar : ScalarAt unit shape.row)
    (validResults : ValidResultsAt executable row shared shape function)
    (validArgs : HList.Holds row args) :
    wp (closureMeaning executable row shared shape function args)
      (fun result _ => shape.Holds result) (fun _ => True) initial :=
  wp_closureMeaning_unseen_at executable natives agrees agree representable function args
    invocable shapeInvocable free shapeFree parametersScalar resultsScalar validResults validArgs
    (fun _ _ holds _ _ _ _ => holds) (fun _ _ _ => trivial)

end

#print axioms invalid_run_refutes_validResults
#print axioms invalid_run_refutes_result_fact
#print axioms validResults_gives_result_fact

end VerexIR.Tests.UnseenResultValidity
