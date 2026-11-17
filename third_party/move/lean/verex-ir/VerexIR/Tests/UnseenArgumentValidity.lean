-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! # The argument validity of an invocation through a dictionary's contract

An invocation of a function value a dictionary's contract covers is taken by
that contract's rule (`MethodContract.invoke`), which owes the validity of
the arguments it passes.  The condition is read by the reader every call's
condition has, and an atom the leaf does not state stays a goal of the
invocation, beside the goal of what follows it.  The control runs the rule
on a leaf whose argument has no fact and reads the goals it leaves.
-/

namespace VerexIR.Tests.UnseenArgumentValidity

open Lean Elab Command Tactic Meta

set_option maxRecDepth 4000

/-- A declared type with one field: its invariant is the unit's, so it is no
consequence of the value. -/
abbrev Guarded : NTy := .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 64 false) .nil)

/-- The invoked value's row: one argument of the declared type. -/
abbrev row : NRow := .cons Guarded .nil
abbrev shape : ResultShape := .none

/-- Runs the invocation rule on the leaf `statement` states and reports the
value it invokes, the goals it leaves to go on with, and the atoms of the
argument validity it owes with the positions of the arguments they belong
to, counting from one. -/
elab "#guard_unseen_goals " name:str " in " statement:term : command => liftTermElabM do
  let statement ← instantiateMVars (← Term.elabType statement)
  Term.synthesizeSyntheticMVarsNoPostponing
  let statement ← whnfR (← instantiateMVars statement)
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, goal) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let report ← IO.mkRef "the rule was not taken"
  let _ ← Tactic.run goal do
    match ← unseenInvocation? goal with
    | none => pure ()
    | some invocation =>
        let mut stated := #[]
        for subgoal in invocation.continuations do
          stated := stated.push
            ((toString (← ppExpr (← instantiateMVars (← subgoal.getType)))).replace "\n" " ")
        let mut owed := #[]
        for (position, atom) in invocation.owed do
          owed := owed.push (position + 1,
            (toString (← ppExpr (← instantiateMVars (← atom.getType)))).replace "\n" " ")
        report.set s!"value `{invocation.value}`, continuations {stated.toList}, \
          owed {owed.toList}"
    setGoals []
  logInfo m!"{name.getString}: {← report.get}"

/-! The argument's fact is not in the leaf's hand: the atom it owes is left
as a goal of the invocation, opened to the clause the declaration states of
it, beside the goal of what the invocation returns.  Two goals is what this
row leaves: a declaration whose own statement reads several clauses or
fields leaves one goal for each of them. -/

/--
info: openAtom: value `function`, continuations [True], owed [(1, Carriers.own { namespaceId := { index := 0 }, structId := 0 } NRow.nil   ((NTy.struct { namespaceId := { index := 0 }, structId := 0 } NRow.nil (NRow.cons (NTy.int 64 false) NRow.nil)).encode     x))]
-/
#guard_msgs in
#guard_unseen_goals "openAtom" in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit) [Skolems unit]
    (shared : Sharing)
    (function : @NTy.carrier Skolems.toCarriers (NTy.function row shared shape.row))
    (x : @NTy.carrier Skolems.toCarriers Guarded)
    (initial : Memory unit)
    (_contract : @MethodContract unit _ executable _ row shared shape function),
    wp (@closureMeaning unit _ executable _ row shared shape
      function (x, ())) (fun _ _ => True) (fun _ => True) initial

/-! ## The key an invocation's argument gives its obligation

The value is named for the message and never for the key, so a leaf's
obligation is selected by the argument's position and not by how the value
prints.  Where the invocation has no call number the key is the position's,
as a call's argument's is: two arguments at one position of two invocations
of the same body share it, and an authored proof selects them together.
-/

/-- info: true -/
#guard_msgs in
#eval (Provenance.invocationArgument "f" none 2).key ==
  (Provenance.invocationArgument "callback" none 2).key

/-- info: true -/
#guard_msgs in
#eval (Provenance.invocationArgument "f" (some 1) 2).key !=
  (Provenance.invocationArgument "f" (some 2) 2).key

/-- info: true -/
#guard_msgs in
#eval (Provenance.invocationArgument "f" none 1).key !=
  (Provenance.invocationArgument "f" none 2).key

/-- info: "argument 2 of the invocation of `f` at call 1" -/
#guard_msgs in
#eval (Provenance.invocationArgument "f" (some 1) 2).describe

end VerexIR.Tests.UnseenArgumentValidity
