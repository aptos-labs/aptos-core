-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types
import Lean

/-! Checked conversion of encodings at concretely instantiated families.
The carriers can be definitionally equal while an enum's transport still
matches on its value. Prove that transport is the identity once, separately
from the source obligations that use the encoding. -/

namespace VerexIR.Proofs.Denote.FamilyTransport

open Lean Meta

/-- Structural transport identities: case analysis is confined to the
transport's input. Collection elements are proved pointwise, without
enumerating the collection or multiplying independent field cases. -/
private partial def identity (goal : MVarId) (fuel : Nat := 128) : MetaM Unit := goal.withContext do
  if fuel == 0 then throwError "family transport exceeds structural depth"
  try goal.refl; return catch _ => pure ()
  let some (type, lhs, rhs) := (← instantiateMVars (← goal.getType)).eq?
    | throwError "expected a family transport identity"
  let type ← whnf type
  if rhs.isFVar && (type.isAppOf ``Sum || type.isAppOf ``Prod ||
      type.isConstOf ``Empty || type.isAppOf ``SpecEntries || type.isAppOf ``SpecVector) then
    for branch in ← goal.cases rhs.fvarId! do identity branch.mvarId (fuel - 1)
    return
  -- Match the map lemma before weak-head reduction can enter Array.map's
  -- implementation. Only its element transport needs a proof.
  if type.isAppOf ``List || type.isAppOf ``Array then
    let theoremName := if type.isAppOf ``List then ``List.map_id'' else ``Array.map_id''
    for child in ← goal.apply (← mkConstWithFreshMVarLevels theoremName) do
      let (_, child) ← child.intro1
      identity child (fuel - 1)
    return
  let lhs ← whnf lhs
  let rhs ← whnf rhs
  let goal ← goal.replaceTargetDefEq (← mkEq lhs rhs)
  try goal.refl; return catch _ => pure ()
  -- `congrN` normally tries every local assumption. These are identities
  -- of the transport itself; searching a source context is both unnecessary
  -- and costly, and would add dependencies to the cached proof.
  let children ← goal.congrN 1 false false
  if children == [goal] then throwError "family transport is not structurally the identity"
  for child in children do identity child (fuel - 1)

/-- Equality of the two encoding functions, with every use discharged by
`NTy.encode_ofSkolem` and a proved identity transport. No source hypothesis
is used. The caller owns the proof's budget and cache scope. -/
def encodingEq (family outer arguments type substituted : Lean.Expr) : MetaM Lean.Expr :=
    withTransparency .default do
  let carrier := mkApp2 (mkConst ``NTy.carrier) family type
  let outerCarrier := mkApp2 (mkConst ``NTy.carrier) outer substituted
  unless ← isDefEq carrier outerCarrier do throwError "family carriers do not convert"
  let pointwise ← withLocalDeclD `value carrier fun value => do
    let transported := mkAppN (mkConst ``NTy.ofSkolem) #[outer, arguments, type, value]
    let same ← mkFreshExprMVar (← mkEq transported value)
    identity same.mvarId!
    let encoded := mkAppN (mkConst ``NTy.encode_ofSkolem) #[outer, arguments, type, value]
    let outerEncode := mkApp2 (mkConst ``NTy.encode) outer substituted
    let proof ← mkEqTrans (← mkEqSymm encoded) (← mkCongrArg outerEncode (← instantiateMVars same))
    mkLambdaFVars #[value] proof
  mkAppM ``funext #[pointwise]

/-- That a transport returns every value of `carrier`: a proof of
`fun value => transport value` equal to the identity, checked structurally. -/
def pointwiseIdentity (carrier : Lean.Expr) (transport : Lean.Expr → Lean.Expr) :
    MetaM Lean.Expr := withTransparency .default do
  let pointwise ← withLocalDeclD `value carrier fun value => do
    let same ← mkFreshExprMVar (← mkEq (transport value) value)
    identity same.mvarId!
    mkLambdaFVars #[value] (← instantiateMVars same)
  mkAppM ``funext #[pointwise]

/-- That carrying a caller's value into the callee's view returns it, at the
caller's `carrier` (`pointwiseIdentity`). -/
def toSkolemIdentity (outer arguments type carrier : Lean.Expr) : MetaM Lean.Expr :=
  pointwiseIdentity carrier fun value =>
    mkAppN (mkConst ``NTy.toSkolem) #[outer, arguments, type, value]

/-! ## Induced families

The families a call's type arguments induce, and the substitution of
spelled types by spelled rows, as the transports evaluate them. -/

/-- The type arguments and outer carriers of the carriers a call's type
arguments induce, in either spelling: as carriers, or as the carriers of an
induced frame. -/
def inducedCarriers? (family : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if family.isAppOfArity ``VerexIR.Proofs.Denote.Carriers.instantiate 2 then
    some (family.getArg! 0, family.getArg! 1)
  else if family.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.toCarriers 2 then
    let frame := family.getArg! 1
    if frame.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.instantiate 3 then
      some (frame.getArg! 1,
        mkApp2 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) (frame.getArg! 0)
          (frame.getArg! 2))
    else none
  else none

mutual
/-- Whether a native type is spelled with constructors and literal type
parameters only, so that its substitution evaluates. -/
partial def spelledNTy (τ : Lean.Expr) : Bool :=
  let τ := τ.consumeMData
  match τ.getAppFn.constName?, τ.getAppArgs with
  | some ``VerexIR.Proofs.Denote.NTy.unit, #[] | some ``VerexIR.Proofs.Denote.NTy.bool, #[]
  | some ``VerexIR.Proofs.Denote.NTy.address, #[] | some ``VerexIR.Proofs.Denote.NTy.signer, #[]
  | some ``VerexIR.Proofs.Denote.NTy.string, #[] | some ``VerexIR.Proofs.Denote.NTy.bytes, #[] => true
  | some ``VerexIR.Proofs.Denote.NTy.int, #[_, _] | some ``VerexIR.Proofs.Denote.NTy.num, #[] => true
  | some ``VerexIR.Proofs.Denote.NTy.param, #[index] => index.nat?.isSome || index.rawNatLit?.isSome
  | some ``VerexIR.Proofs.Denote.NTy.tuple, #[row] => spelledNRow row
  | some ``VerexIR.Proofs.Denote.NTy.struct, #[_, arguments, row] =>
      spelledNRow arguments && spelledNRow row
  | some ``VerexIR.Proofs.Denote.NTy.enum, #[_, arguments, _, rows, _] =>
      spelledNRow arguments && spelledNRows rows
  | some ``VerexIR.Proofs.Denote.NTy.vector, #[element] => spelledNTy element
  | some ``VerexIR.Proofs.Denote.NTy.table, #[key, value] => spelledNTy key && spelledNTy value
  | some ``VerexIR.Proofs.Denote.NTy.ref, #[referent] => spelledNTy referent
  | some ``VerexIR.Proofs.Denote.NTy.ghost, #[slot] => spelledNTy slot
  | some ``VerexIR.Proofs.Denote.NTy.function, #[parameters, _, results] =>
      spelledNRow parameters && spelledNRow results
  | _, _ => false

partial def spelledNRow (row : Lean.Expr) : Bool :=
  let row := row.consumeMData
  match row.getAppFn.constName?, row.getAppArgs with
  | some ``VerexIR.Proofs.Denote.NRow.nil, #[] => true
  | some ``VerexIR.Proofs.Denote.NRow.cons, #[τ, rest] => spelledNTy τ && spelledNRow rest
  | _, _ => false

partial def spelledNRows (rows : Lean.Expr) : Bool :=
  let rows := rows.consumeMData
  match rows.getAppFn.constName?, rows.getAppArgs with
  | some ``VerexIR.Proofs.Denote.NRows.nil, #[] => true
  | some ``VerexIR.Proofs.Denote.NRows.cons, #[row, rest] => spelledNRow row && spelledNRows rest
  | _, _ => false
end

/-- The row a type-argument transport substitutes, when it is spelled. -/
def substitutionRow? (θ : Lean.Expr) : Option Lean.Expr :=
  if θ.isAppOfArity ``Subtype.mk 4 then some (θ.getArg! 2) else none

/-- The entry of a spelled row at a position, as `NRow.getD` reads it. -/
partial def rowEntry? (row : Lean.Expr) (index : Nat) : Option Lean.Expr :=
  if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 then
    if index == 0 then some (row.getArg! 0) else rowEntry? (row.getArg! 1) (index - 1)
  else none

mutual
/-- The substitution of a spelled type by a spelled row, evaluated to a
spelling, as `NTy.subst` defines it. -/
partial def substituteNTy (θ : Lean.Expr) (τ : Lean.Expr) : Option Lean.Expr := do
  let τ := τ.consumeMData
  let args := τ.getAppArgs
  match τ.getAppFn.constName? with
  | some ``VerexIR.Proofs.Denote.NTy.param =>
      let index ← (args[0]?).bind fun index => index.nat? <|> index.rawNatLit?
      pure ((rowEntry? θ index).getD τ)
  | some ``VerexIR.Proofs.Denote.NTy.tuple =>
      pure (mkApp τ.getAppFn (← substituteNRow θ (← args[0]?)))
  | some ``VerexIR.Proofs.Denote.NTy.struct =>
      pure (mkApp3 τ.getAppFn (← args[0]?) (← substituteNRow θ (← args[1]?))
        (← substituteNRow θ (← args[2]?)))
  | some ``VerexIR.Proofs.Denote.NTy.enum =>
      pure (mkApp5 τ.getAppFn (← args[0]?) (← substituteNRow θ (← args[1]?)) (← args[2]?)
        (← substituteNRows θ (← args[3]?)) (← args[4]?))
  | some ``VerexIR.Proofs.Denote.NTy.vector | some ``VerexIR.Proofs.Denote.NTy.ref |
      some ``VerexIR.Proofs.Denote.NTy.ghost =>
      pure (mkApp τ.getAppFn (← substituteNTy θ (← args[0]?)))
  | some ``VerexIR.Proofs.Denote.NTy.table =>
      pure (mkApp2 τ.getAppFn (← substituteNTy θ (← args[0]?)) (← substituteNTy θ (← args[1]?)))
  | some ``VerexIR.Proofs.Denote.NTy.function =>
      pure (mkApp3 τ.getAppFn (← substituteNRow θ (← args[0]?)) (← args[1]?)
        (← substituteNRow θ (← args[2]?)))
  | _ => if spelledNTy τ then some τ else none

partial def substituteNRow (θ : Lean.Expr) (row : Lean.Expr) : Option Lean.Expr := do
  let row := row.consumeMData
  if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 then
    pure (mkApp2 row.getAppFn (← substituteNTy θ (row.getArg! 0)) (← substituteNRow θ (row.getArg! 1)))
  else if row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then some row
  else none

partial def substituteNRows (θ : Lean.Expr) (rows : Lean.Expr) : Option Lean.Expr := do
  let rows := rows.consumeMData
  if rows.isAppOfArity ``VerexIR.Proofs.Denote.NRows.cons 2 then
    pure (mkApp2 rows.getAppFn (← substituteNRow θ (rows.getArg! 0))
      (← substituteNRows θ (rows.getArg! 1)))
  else if rows.isConstOf ``VerexIR.Proofs.Denote.NRows.nil then some rows
  else none
end

/-- The families of indices into native types: a variable of a row, the
variables of a row, a variant of an enum, a field the variants share. -/
def indexFamilies : List Name :=
  -- `Vars`, `Which` and `Choices` are the denotation's, later than this module.
  [``Var, `VerexIR.Proofs.Denote.Vars, `VerexIR.Proofs.Denote.Which,
    `VerexIR.Proofs.Denote.Choices]

/-- An index into native types (`indexFamilies`), a chain of constructors,
with the types it is indexed by substituted by `row`; none for any other
term, or where a type does not substitute. -/
partial def respellIndex? (env : Environment) (row e : Lean.Expr) : Option Lean.Expr := do
  let e := e.consumeMData
  let .const name _ := e.getAppFn | none
  guard (env.isConstructor name && indexFamilies.contains name.getPrefix)
  let args ← e.getAppArgs.mapM fun arg => do
    let arg := arg.consumeMData
    let some head := arg.getAppFn.constName? | pure arg
    unless env.isConstructor head do return arg
    let kind := head.getPrefix
    if kind == ``NTy then substituteNTy row arg
    else if kind == ``NRow then substituteNRow row arg
    else if kind == ``NRows then substituteNRows row arg
    else if indexFamilies.contains kind then respellIndex? env row arg
    else pure arg
  return mkAppN e.getAppFn args

/-- The induced family an operation is at, from its arguments: the carriers
it takes first, or the frame it takes after its unit. Its position, the type
arguments, and the outer family. -/
def operationFamily? (args : Array Lean.Expr) : Option (Nat × Lean.Expr × Lean.Expr) :=
  match args[0]?.bind (fun family => inducedCarriers? family.consumeMData) with
  | some (θ, outer) => some (0, θ, outer)
  | none => do
      let frame := (← args[1]?).consumeMData
      guard (frame.isAppOfArity ``Skolems.instantiate 3)
      pure (1, frame.getArg! 1, frame.getArg! 2)

/-- A native operation at an induced family whose type arguments are
spelled, at the outer family with each type and index it takes substituted,
and the number of its arguments up to the last type or index; none where a
type does not substitute. At spelled types the two are definitionally equal
where the operation reduces on its types and indices (`respellOperation?`
checks it on those arguments, not on the values, which it would evaluate). -/
def respelledOperation? (t : Lean.Expr) : MetaM (Option (Lean.Expr × Nat)) := do
  let some head := t.getAppFn.constName? | return none
  unless (`VerexIR.Proofs.Denote).isPrefixOf head do return none
  let args := t.getAppArgs
  let some (position, θ, outer) := operationFamily? args | return none
  let some row := substitutionRow? θ | return none
  let env ← getEnv
  let spelledType (e : Lean.Expr) : Bool := match e.consumeMData.getAppFn.constName? with
    | some name => env.isConstructor name && [``NTy, ``NRow, ``NRows].contains name.getPrefix
    | none => false
  let mut rebuiltArgs := (args.extract 0 position).push outer
  let mut typed := position + 1
  for arg in args.extract (position + 1) args.size do
    if spelledType arg then
      let kind := arg.consumeMData.getAppFn.constName!.getPrefix
      let substituted := if kind == ``NTy then substituteNTy row arg
        else if kind == ``NRow then substituteNRow row arg
        else substituteNRows row arg
      let some substituted := substituted | return none
      rebuiltArgs := rebuiltArgs.push substituted
      typed := rebuiltArgs.size
    else if let some index := respellIndex? env row arg then
      rebuiltArgs := rebuiltArgs.push index
      typed := rebuiltArgs.size
    else rebuiltArgs := rebuiltArgs.push arg
  return some (mkAppN t.getAppFn rebuiltArgs, typed)

/-- `respelledOperation?` where the two spellings convert. -/
def respellOperation? (t : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let some (respelled, typed) ← respelledOperation? t | return none
  let operation := mkAppN t.getAppFn (t.getAppArgs.extract 0 typed)
  if operation.hasLooseBVars then return none
  unless ← isDefEq operation (mkAppN respelled.getAppFn (respelled.getAppArgs.extract 0 typed)) do
    return none
  return some respelled

/-- A term with the types and defaults it spells at an induced family
spelled at the outer family, as the caller states them: carriers, rows,
variants, result shapes, and defaults (`NTy.inhabitantOf`), in types and
instances too, which simp leaves as they are. At spelled types the two
spellings are definitionally equal: the induced family's parameter is the
outer family's substituted type. -/
def respellInducedFamily (e : Lean.Expr) : MetaM Lean.Expr :=
  Meta.transform e (post := fun e => do
    let .const name _ := e.getAppFn | return .continue
    let args := e.getAppArgs
    -- A substitution by a spelled row, evaluated; a parameter's value read in
    -- the outer view, the value.
    let spelled (row : Lean.Expr) : MetaM (Option Lean.Expr) := do
      let row := row.consumeMData
      if row.isAppOfArity ``Subtype.val 3 then
        return substitutionRow? (← whnfR (row.getArg! 2))
      return if spelledNRow row then some row else none
    match name, args with
    | ``NTy.subst, #[row, τ] =>
        if let some row ← spelled row then
          if let some τ' := substituteNTy row τ then return .done τ'
        return .continue
    | ``NRow.subst, #[row, fields] =>
        if let some row ← spelled row then
          if let some fields' := substituteNRow row fields then return .done fields'
        return .continue
    | ``NRows.subst, #[row, rows] =>
        if let some row ← spelled row then
          if let some rows' := substituteNRows row rows then return .done rows'
        return .continue
    | ``NTy.ofSkolem, #[_, _, τ, value] =>
        if τ.consumeMData.isAppOfArity ``NTy.param 1 then return .done value
        return .continue
    | _, _ => pure ()
    let some family := args[0]? | return .continue
    let some (θ, outer) := inducedCarriers? family | return .continue
    let some row := substitutionRow? θ | return .continue
    let carrier (τ : Lean.Expr) : Option Lean.Expr :=
      (substituteNTy row τ).map (mkApp2 (mkConst ``NTy.carrier) outer)
    let respelled : Option Lean.Expr ← match name, args with
      | ``NTy.carrier, #[_, τ] => pure (carrier τ)
      | ``HList, #[_, fields] =>
          pure ((substituteNRow row fields).map (mkApp2 (mkConst ``HList) outer))
      | ``variantCarrier, #[_, names, rows] =>
          pure ((substituteNRows row rows).map (mkApp3 (mkConst ``variantCarrier) outer names))
      | ``Carriers.carrier, #[_, index] =>
          pure do carrier (← rowEntry? row (← index.nat? <|> index.rawNatLit?))
      | ``ResultShape.carrier, #[_, shape] => do
          match (← whnfD shape).getAppFn.constName?, (← whnfD shape).getAppArgs with
          | some ``ResultShape.none, #[] => pure (some (mkConst ``Unit))
          | some ``ResultShape.one, #[τ] => pure (carrier τ)
          | _, _ => pure none
      -- A parameter's order is the outer family's at its argument
      -- (`Carriers.instantiate`, `NTy.reads`), at a constructor type the
      -- order it reduces to.
      | ``NTy.readOrder, #[_, rank, τ] => do
          let some substituted := substituteNTy row τ | pure none
          let order := mkApp3 (mkConst ``NTy.readOrder) outer rank substituted
          let env ← getEnv
          let constructor := match substituted.getAppFn with
            | .const head _ => env.isConstructor head && head != ``NTy.param
            | _ => false
          if constructor then
            let reduced ← whnfR order
            if reduced.isConstOf ``VerexIR.SpecMap.intOrder || reduced.isAppOf ``NTy.keyOrder then
              pure (some reduced)
            else pure (some order)
          else pure (some order)
      | ``NTy.inhabitantOf, #[_, τ, _] => do
          let some substituted := substituteNTy row τ | pure none
          let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
            (← mkEq (mkApp (mkConst ``NTy.inhabitable) substituted) (mkConst ``Bool.true))
          pure (some (mkApp3 (mkConst ``NTy.inhabitantOf) outer substituted decided))
      | _, _ => pure none
    match respelled with
    | some value => return .done value
    | none =>
    -- Any other native operation at the induced family, such as a field's
    -- selection, at the outer family.
    match ← respellOperation? e with
    | some value => return .done value
    | none => return .continue)

end VerexIR.Proofs.Denote.FamilyTransport
