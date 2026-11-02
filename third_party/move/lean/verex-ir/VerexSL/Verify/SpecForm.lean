-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.PrettyPrinter.Delaborator
import VerexIR.Proofs.Maps.Model
import VerexIR.Validation.TableRoles

/-!
# Spec form

The native vocabulary of a goal printed as the specification writes it
(`doc/plan/proof-language.md`, "Messages and templates"): the clause
translator run backwards. A map's reading prints under its owner's role
function (`SpecMap.keyAt m i` as `spec_key_at(m, i)`, `SpecMap.hasKey m k =
true` as `spec_contains_key(m, k)`), an integer's value as the integer.
The delaborators run where `verex.specForm` is set, which a message of the
proof language sets; a successful check prints nothing.
-/

namespace VerexSL.SpecForm

open Lean PrettyPrinter Delaborator SubExpr
open VerexIR (MapSpecRole)

register_option verex.specForm : Bool := {
  defValue := false
  descr := "print goals' native vocabulary in specification form" }

register_option verex.specRoles : String := {
  defValue := ""
  descr := "the role functions of the module's maps, `role=name` pairs separated by `,`, \
    each role by its source name (`map_spec_key_at=spec_key_at`)" }

/-- A call as the specification writes it, `f(a, …)`: printed only. -/
syntax:max (name := specFormCall) ident noWs "(" term,* ")" : term

/-- The name a role prints under: the role function of the module's maps
(`verex.specRoles`), else the role's source name. -/
private def roleName (role : MapSpecRole) : DelabM String := do
  let table := verex.specRoles.get (← getOptions)
  let bound := (table.splitOn ",").findSome? fun pair =>
    match pair.splitOn "=" with
    | [key, name] => if key == role.name then some name else none
    | _ => none
  return bound.getD role.name

/-- The last `count` arguments of the application under the cursor, delaborated. -/
private def lastArguments (count : Nat) : DelabM (Array Term) := do
  let arguments := (← getExpr).getAppNumArgs
  if arguments < count then failure
  (Array.range count).mapM fun index => withNaryArg (arguments - count + index) delab

/-- A role's application in spec form, its explicit arguments the last
`count`. -/
private def roleCall (role : MapSpecRole) (count : Nat) : Delab := do
  unless verex.specForm.get (← getOptions) do failure
  let arguments ← lastArguments count
  let name := mkIdent (Name.mkSimple (← roleName role))
  `($name:ident($arguments,*))

@[delab app.VerexIR.SpecMap.keyAt] def delabKeyAt : Delab := roleCall .keyAt 2
@[delab app.VerexIR.SpecMap.valueAt] def delabValueAt : Delab := roleCall .get 2
@[delab app.VerexIR.SpecMap.hasKey] def delabHasKey : Delab := roleCall .hasKey 2
@[delab app.VerexIR.SpecMap.size] def delabSize : Delab := roleCall .len 1
@[delab app.VerexIR.SpecMap.rank] def delabRank : Delab := roleCall .rank 2
@[delab app.VerexIR.SpecMap.update] def delabUpdate : Delab := roleCall .set 3
@[delab app.VerexIR.SpecMap.remove] def delabRemove : Delab := roleCall .del 2

/-- An equation as the specification's `==`; a Boolean's truth as the
Boolean: `b = true` as `b`, `b = false` as `!b`. -/
@[delab app.Eq] def delabEquation : Delab := do
  unless verex.specForm.get (← getOptions) do failure
  let e ← getExpr
  unless e.isAppOfArity ``Eq 3 do failure
  let value := e.getArg! 2
  if (e.getArg! 0).isConstOf ``Bool && value.isConstOf ``Bool.true then withNaryArg 1 delab
  else if (e.getArg! 0).isConstOf ``Bool && value.isConstOf ``Bool.false then do
    let operand ← withNaryArg 1 delab
    `(!$operand)
  else
    let left ← withNaryArg 1 delab
    let right ← withNaryArg 2 delab
    `($left == $right)

/-- A negated equation as the specification's `!=`. -/
@[delab app.Not] def delabDisequation : Delab := do
  unless verex.specForm.get (← getOptions) do failure
  let e ← getExpr
  unless e.isAppOfArity ``Not 1 && (e.getArg! 0).isAppOfArity ``Eq 3 do failure
  let left ← withNaryArg 0 (withNaryArg 1 delab)
  let right ← withNaryArg 0 (withNaryArg 2 delab)
  `($left != $right)

/-- An integer's value as the integer (`x.val` as `x`). -/
@[delab app.VerexIR.SpecInt.val] def delabValue : Delab := do
  unless verex.specForm.get (← getOptions) do failure
  unless (← getExpr).getAppNumArgs == 3 do failure
  withNaryArg 2 delab

/-- The options that print in spec form, the maps' roles under `roles`. -/
def options (roles : String) (current : Options) : Options :=
  (current.set verex.specForm.name true).set verex.specRoles.name roles

end VerexSL.SpecForm
