-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Tests.ConstructorEquations

open VerexIR.Tests.ConstructorEquations

-- Equation discovery after import cannot rely on a local elaboration cache.
attribute [vir_denote] Term.eval Arguments.eval

open Lean Meta in
run_meta do
  let some equations ← getEqnsFor? ``Term.eval | throwError "missing imported equations"
  unless equations == #[``Term.eval.eq_1, ``Term.eval.eq_2] do
    throwError "imported equation inventory differs"
  unless !(← getEnv).contains (mkEqLikeNameFor (← getEnv) ``Term.eval unfoldThmSuffix) do
    throwError "import generated the symbolic unfolding theorem"
  for equation in equations do
    if ← isBackwardRflTheorem equation then
      throwError "equation evidence would be discarded by simplification"
    unless (← collectAxioms equation).isEmpty do
      throwError "imported constructor equation depends on axioms"

example (arguments : Arguments) : (Term.sum arguments).eval = arguments.eval := by
  simp only [Term.eval.eq_2]
