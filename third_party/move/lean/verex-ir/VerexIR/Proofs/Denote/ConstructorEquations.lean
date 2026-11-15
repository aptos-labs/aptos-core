-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
module
public import Lean
import all Lean.Elab.PreDefinition.Eqns

/-- Specialize source equations without first constructing a symbolic
structural unfolding theorem. Proofs are supplied and checked separately. -/
public def VerexIR.DenoteEquationTypes (names : Array Lean.Name) (goal : Lean.MVarId) :
    Lean.MetaM (Array Lean.Expr) := Lean.Elab.Eqns.mkEqnTypes names goal

/-- Use the standard equation cleanup so discarded split hypotheses do not
become unnecessary simplifier arguments. -/
public def VerexIR.cleanDenoteEquation (type value : Lean.Expr) :
    Lean.CoreM (Lean.Expr × Lean.Expr) :=
  Lean.Elab.Eqns.removeUnusedEqnHypotheses type value
