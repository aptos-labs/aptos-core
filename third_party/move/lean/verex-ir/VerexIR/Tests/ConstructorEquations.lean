-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Attr

namespace VerexIR.Tests.ConstructorEquations

-- Dependent, mutually recursive evaluators exercise the same equation path as
-- Term/Args without requiring the full denotation module to build first.
mutual
inductive Term : Type → Type where
  | literal (value : Nat) : Term Nat
  | sum (arguments : Arguments) : Term Nat
inductive Arguments where
  | nil : Arguments
  | cons (head : Term Nat) (tail : Arguments) : Arguments
end

mutual
def Term.eval : {α : Type} → Term α → α
  | _, .literal value => value
  | _, .sum arguments => arguments.eval
def Arguments.eval : Arguments → Nat
  | .nil => 0
  | .cons head tail => head.eval + tail.eval
end

vir_denote_constructor_equations Term.eval Arguments.eval
-- Registering the definitions must reuse the equations, rather than generating
-- the expensive symbolic unfolding theorem afterward.
attribute [vir_denote] Term.eval Arguments.eval

example (head : Term Nat) (tail : Arguments) :
    (Term.sum (.cons head tail)).eval = head.eval + tail.eval := by
  simp only [Term.eval.eq_2, Arguments.eval.eq_2]

open Lean Meta in
run_meta do
  for name in #[``Term.eval, ``Arguments.eval] do
    let some equations ← getEqnsFor? name | throwError "missing constructor equations"
    unless equations.size == 2 do throwError "unexpected equation count"
    unless !(← getEnv).contains (mkEqLikeNameFor (← getEnv) name unfoldThmSuffix) do
      throwError "symbolic unfolding theorem was generated"
    for equation in equations do
      if ← isBackwardRflTheorem equation then
        throwError "equation evidence would be discarded by simplification"
      unless (← collectAxioms equation).isEmpty do
        throwError "constructor equation depends on axioms"

end VerexIR.Tests.ConstructorEquations
