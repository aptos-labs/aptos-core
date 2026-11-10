-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.NominalQuote
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
  let nonempty ← if family.isAppOfArity ``Carriers.instantiate 3 then
      pure (family.getArg! 2)
    else synthInstance (mkApp (mkConst ``CarriersNonempty) outer)
  let pointwise ← withLocalDeclD `value carrier fun value => do
    let transported ← mkAppOptM ``NTy.ofSkolem
      #[some outer, some nonempty, some arguments, some type, some value]
    let same ← mkFreshExprMVar (← mkEq transported value)
    identity same.mvarId!
    let encoded ← mkAppOptM ``NTy.encode_ofSkolem
      #[some outer, some nonempty, some arguments, some type, some value]
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
def toSkolemIdentity (outer nonempty arguments type carrier : Lean.Expr) : MetaM Lean.Expr :=
  pointwiseIdentity carrier fun value =>
    mkAppN (mkConst ``NTy.toSkolem) #[outer, nonempty, arguments, type, value]

/-! ## Induced families

The families a call's type arguments induce, and the substitution of
spelled types by spelled rows, as the transports evaluate them. -/

/-- The default of a type at the family a call's type arguments induce is
the outer family's default of the substituted type: the two are one value,
the outer family's, carried into the callee's view
(`NTy.inhabitantOf_instantiate`), and carrying returns the value it carries
(`NTy.toSkolem_heq`).

The equality is heterogeneous because the statement holds at every type,
while the two defaults have one type only where the type's carrier does not
read the family — the families themselves are never equal, and an operation
at one of them is the operation at the other exactly where it reads the
family through its type.  A step that exchanges the families must therefore
check that much and take the equality from here, rather than claim the
exchange as a conversion. -/
theorem inhabitantOf_instantiate_heq {Θ : Carriers} [CarriersNonempty Θ]
    (θ : TypeArgs) (τ : NTy) (inhabitable : τ.inhabitable = true)
    (substituted : (τ.subst θ.1).inhabitable = true) :
    HEq
      (@NTy.inhabitantOf (Carriers.instantiate θ Θ) (CarriersNonempty.instantiate θ Θ) τ
        inhabitable)
      (@NTy.inhabitantOf Θ _ (τ.subst θ.1) substituted) := by
  rw [NTy.inhabitantOf_instantiate]
  exact NTy.toSkolem_heq θ τ _

/-- The type arguments and outer carriers of the carriers a call's type
arguments induce, in either spelling: as carriers, or as the carriers of an
induced frame. -/
def inducedCarriers? (family : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  -- An induced family takes the outer family's inhabitants as well
  -- (`Carriers.instantiate`), so it is spelled with three arguments.
  if family.isAppOfArity ``VerexIR.Proofs.Denote.Carriers.instantiate 3 ||
      family.isAppOfArity ``VerexIR.Proofs.Denote.Carriers.instantiate 2 then
    some (family.getArg! 0, family.getArg! 1)
  else if family.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.toCarriers 3 then
    let frame := family.getArg! 2
    if frame.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.instantiate 4 then
      some (frame.getArg! 2,
        mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) (frame.getArg! 0)
          (frame.getArg! 1) (frame.getArg! 3))
    else none
  else none

/-- Only registered, fully applied descriptor heads are nominal type spellings.
The arguments remain explicit; inspecting them never unfolds their field layouts. -/
private def nominalArguments? (env : Option Environment) (τ : Lean.Expr) :
    Option (Array Lean.Expr) := do
  guard (isNominalType (← env) τ)
  return τ.getAppArgs

mutual
/-- Whether a native type is spelled with constructors and literal type
parameters or registered nominal descriptors, so that its substitution evaluates.
The optional environment enables descriptors without changing structural-only callers. -/
partial def spelledNTy (τ : Lean.Expr) (env : Option Environment := none) : Bool :=
  let τ := τ.consumeMData
  match τ.getAppFn.constName?, τ.getAppArgs with
  | some ``VerexIR.Proofs.Denote.NTy.unit, #[] | some ``VerexIR.Proofs.Denote.NTy.bool, #[]
  | some ``VerexIR.Proofs.Denote.NTy.address, #[] | some ``VerexIR.Proofs.Denote.NTy.signer, #[]
  | some ``VerexIR.Proofs.Denote.NTy.string, #[] | some ``VerexIR.Proofs.Denote.NTy.bytes, #[] => true
  | some ``VerexIR.Proofs.Denote.NTy.int, #[_, _] | some ``VerexIR.Proofs.Denote.NTy.num, #[] => true
  | some ``VerexIR.Proofs.Denote.NTy.param, #[index] => index.nat?.isSome || index.rawNatLit?.isSome
  | some ``VerexIR.Proofs.Denote.NTy.tuple, #[row] => spelledNRow row env
  | some ``VerexIR.Proofs.Denote.NTy.struct, #[_, arguments, row] =>
      spelledNRow arguments env && spelledNRow row env
  | some ``VerexIR.Proofs.Denote.NTy.enum, #[_, arguments, _, rows, _] =>
      spelledNRow arguments env && spelledNRows rows env
  | some ``VerexIR.Proofs.Denote.NTy.vector, #[element] => spelledNTy element env
  | some ``VerexIR.Proofs.Denote.NTy.table, #[key, value] => spelledNTy key env && spelledNTy value env
  | some ``VerexIR.Proofs.Denote.NTy.ref, #[referent] => spelledNTy referent env
  | some ``VerexIR.Proofs.Denote.NTy.ghost, #[slot] => spelledNTy slot env
  | some ``VerexIR.Proofs.Denote.NTy.function, #[parameters, _, results] =>
      spelledNRow parameters env && spelledNRow results env
  | _, _ => match nominalArguments? env τ with
    | some arguments => arguments.all fun argument => spelledNTy argument env
    | none => false

partial def spelledNRow (row : Lean.Expr) (env : Option Environment := none) : Bool :=
  let row := row.consumeMData
  match row.getAppFn.constName?, row.getAppArgs with
  | some ``VerexIR.Proofs.Denote.NRow.nil, #[] => true
  | some ``VerexIR.Proofs.Denote.NRow.cons, #[τ, rest] => spelledNTy τ env && spelledNRow rest env
  | _, _ => false

partial def spelledNRows (rows : Lean.Expr) (env : Option Environment := none) : Bool :=
  let rows := rows.consumeMData
  match rows.getAppFn.constName?, rows.getAppArgs with
  | some ``VerexIR.Proofs.Denote.NRows.nil, #[] => true
  | some ``VerexIR.Proofs.Denote.NRows.cons, #[row, rest] => spelledNRow row env && spelledNRows rest env
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

/-- A parameter lookup can use its default only after reaching the literal
end of the row. An unknown tail is not evidence that the entry is absent. -/
private partial def parameterAt? (row : Lean.Expr) (index : Nat) (fallback : Lean.Expr) :
    Option Lean.Expr :=
  if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 then
    if index == 0 then some (row.getArg! 0)
    else parameterAt? (row.getArg! 1) (index - 1) fallback
  else if row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then some fallback
  else none

mutual
/-- The substitution of a spelled type by a spelled row, evaluated to a
spelling, as `NTy.subst` defines it. A nominal descriptor keeps its head and
substitutes only its explicit type arguments, including phantom arguments. -/
partial def substituteNTy (θ : Lean.Expr) (τ : Lean.Expr)
    (env : Option Environment := none) : Option Lean.Expr := do
  let τ := τ.consumeMData
  let args := τ.getAppArgs
  match τ.getAppFn.constName? with
  | some ``VerexIR.Proofs.Denote.NTy.param =>
      let index ← (args[0]?).bind fun index => index.nat? <|> index.rawNatLit?
      parameterAt? θ index τ
  | some ``VerexIR.Proofs.Denote.NTy.tuple =>
      pure (mkApp τ.getAppFn (← substituteNRow θ (← args[0]?) env))
  | some ``VerexIR.Proofs.Denote.NTy.struct =>
      pure (mkApp3 τ.getAppFn (← args[0]?) (← substituteNRow θ (← args[1]?) env)
        (← substituteNRow θ (← args[2]?) env))
  | some ``VerexIR.Proofs.Denote.NTy.enum =>
      pure (mkApp5 τ.getAppFn (← args[0]?) (← substituteNRow θ (← args[1]?) env) (← args[2]?)
        (← substituteNRows θ (← args[3]?) env) (← args[4]?))
  | some ``VerexIR.Proofs.Denote.NTy.vector | some ``VerexIR.Proofs.Denote.NTy.ref |
      some ``VerexIR.Proofs.Denote.NTy.ghost =>
      pure (mkApp τ.getAppFn (← substituteNTy θ (← args[0]?) env))
  | some ``VerexIR.Proofs.Denote.NTy.table =>
      pure (mkApp2 τ.getAppFn (← substituteNTy θ (← args[0]?) env) (← substituteNTy θ (← args[1]?) env))
  | some ``VerexIR.Proofs.Denote.NTy.function =>
      pure (mkApp3 τ.getAppFn (← substituteNRow θ (← args[0]?) env) (← args[1]?)
        (← substituteNRow θ (← args[2]?) env))
  | _ =>
      if let some arguments := nominalArguments? env τ then
        return mkAppN τ.getAppFn (← arguments.mapM fun argument => substituteNTy θ argument env)
      else if spelledNTy τ env then some τ else none

partial def substituteNRow (θ : Lean.Expr) (row : Lean.Expr)
    (env : Option Environment := none) : Option Lean.Expr := do
  let row := row.consumeMData
  if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 then
    pure (mkApp2 row.getAppFn (← substituteNTy θ (row.getArg! 0) env)
      (← substituteNRow θ (row.getArg! 1) env))
  else if row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then some row
  else none

partial def substituteNRows (θ : Lean.Expr) (rows : Lean.Expr)
    (env : Option Environment := none) : Option Lean.Expr := do
  let rows := rows.consumeMData
  if rows.isAppOfArity ``VerexIR.Proofs.Denote.NRows.cons 2 then
    pure (mkApp2 rows.getAppFn (← substituteNRow θ (rows.getArg! 0) env)
      (← substituteNRows θ (rows.getArg! 1) env))
  else if rows.isConstOf ``VerexIR.Proofs.Denote.NRows.nil then some rows
  else none
end

/-- The families of indices into native types: a variable of a row, the
variables of a row, a variant of an enum, a field the variants share. -/
def indexFamilies : List Name :=
  -- `Vars`, `Which` and `Choices` are the denotation's, later than this module.
  [``Var, `VerexIR.Proofs.Denote.Vars, `VerexIR.Proofs.Denote.Which,
    `VerexIR.Proofs.Denote.Choices]

/-- Substitute the explicit types and constructor fields of an index together.
`Vars.cons` carries its target in `Option (Var Γ σ)`: both the wrapper's
explicit type and the contained variable must use the substituted context,
even for `none`. Other constructor payloads (such as variant names) are inert. -/
private partial def respellIndexArgument? (env : Environment) (row e : Lean.Expr) :
    Option Lean.Expr := do
  let e := e.consumeMData
  let some head := e.getAppFn.constName? | pure e
  if isNominalType env e then return ← substituteNTy row e (some env)
  let constructor := env.isConstructor head
  let kind := head.getPrefix
  if constructor && kind == ``NTy then return ← substituteNTy row e (some env)
  if constructor && kind == ``NRow then return ← substituteNRow row e (some env)
  if constructor && kind == ``NRows then return ← substituteNRows row e (some env)
  let optionalIndex := (head == ``Option.some || head == ``Option.none) &&
    (e.getAppArgs[0]?.bind fun type => type.consumeMData.getAppFn.constName?).any indexFamilies.contains
  if indexFamilies.contains head || (constructor && indexFamilies.contains kind) || optionalIndex then
    let mut type := (← env.find? head).type
    let mut args := #[]
    for arg in e.getAppArgs do
      let .forallE _ domain body _ := type | none
      -- An opaque dependent field cannot be moved to a substituted index.
      -- Decline the whole chain, rather than changing its type annotation
      -- while keeping an old-context value (including an optional target).
      if domain.getAppFn.constName?.any indexFamilies.contains then
        let some name := arg.consumeMData.getAppFn.constName? | none
        guard (env.isConstructor name && indexFamilies.contains name.getPrefix)
      else if domain.isAppOfArity ``Option 1 &&
          domain.appArg!.getAppFn.constName?.any indexFamilies.contains then
        guard (arg.consumeMData.isAppOfArity ``Option.some 2 ||
          arg.consumeMData.isAppOfArity ``Option.none 1)
      args := args.push (← respellIndexArgument? env row arg)
      type := body.instantiate1 arg
    return mkAppN e.getAppFn args
  return e

/-- An index into native types (`indexFamilies`), a chain of constructors,
with the types it is indexed by substituted by `row`; none for any other
term, or where a type does not substitute. -/
def respellIndex? (env : Environment) (row e : Lean.Expr) : Option Lean.Expr := do
  let e := e.consumeMData
  let .const name _ := e.getAppFn | none
  guard (env.isConstructor name && indexFamilies.contains name.getPrefix)
  respellIndexArgument? env row e

/-- The induced representation family an operation is at: native carriers,
their plain-family view, or the frame after its unit and function values.
Its position, type arguments, and corresponding outer representation family. -/
def operationFamily? (args : Array Lean.Expr) : Option (Nat × Lean.Expr × Lean.Expr) :=
  let nativeFamily? := args[0]?.bind fun family => do
    let family := family.consumeMData
    if family.isAppOfArity ``Carriers.toPlain 1 then
      let (θ, outer) ← inducedCarriers? family.appArg!.consumeMData
      return (θ, mkApp (mkConst ``Carriers.toPlain) outer)
    inducedCarriers? family
  match nativeFamily? with
  | some (θ, outer) => some (0, θ, outer)
  | none => do
      let frame := (← args[2]?).consumeMData
      guard (frame.isAppOfArity ``Skolems.instantiate 4)
      pure (2, frame.getArg! 2, frame.getArg! 3)

/-- A native operation at an induced family whose type arguments are
spelled, at the outer family with each type and index it takes substituted,
and the number of its arguments up to the last type or index; none where a
type does not substitute, and none where an argument carries the family's
inhabitants and the outer family has none of its own. At spelled types the
two are definitionally equal where the operation reduces on its types and
indices (`respellOperation?` checks it on those arguments and on the
inhabitants, not on the values, which it would evaluate). -/
def respelledOperation? (t : Lean.Expr) : MetaM (Option (Lean.Expr × Nat)) := do
  let some head := t.getAppFn.constName? | return none
  unless (`VerexIR.Proofs.Denote).isPrefixOf head do return none
  -- These constructors name a family, rather than an operation at one.
  -- Induced and outer families remain distinct; their indexed native
  -- representations are what can acquire the substituted spelling.
  if head == ``Carriers.toPlain || head == ``Skolems.toCarriers then return none
  let args := t.getAppArgs
  let some (position, θ, outer) := operationFamily? args | return none
  let some row := substitutionRow? θ | return none
  let env ← getEnv
  let spelledType (e : Lean.Expr) : Bool := match e.consumeMData.getAppFn.constName? with
    | some name => isNominalType env e ||
        (env.isConstructor name && [``NTy, ``NRow, ``NRows].contains name.getPrefix)
    | none => false
  -- An instance of the family's inhabitants (`CarriersNonempty`) belongs to
  -- the family it was synthesized for: its type names that family, and the
  -- two families are equal under the application they instantiate, not as
  -- terms.  An argument carrying the inner family's inhabitants is
  -- therefore replaced by the outer family's own, synthesized here, as
  -- every transport of a value takes them (`Solve.transportAt?`); the
  -- operation is not respelled where the outer family has none, so that no
  -- application is built at two families.
  let dependentAtOuter? (arg : Lean.Expr) : MetaM (Option (Option Lean.Expr)) := do
    if arg.hasLooseBVars then return none
    let type ← whnfR (← inferType arg)
    -- A recognized index whose constructor chain could not be substituted
    -- cannot remain beside the operation's already-substituted rows. This
    -- also catches an opaque index supplied directly as an argument.
    if type.getAppFn.constName?.any indexFamilies.contains then return some none
    let some family := type.app1? ``CarriersNonempty | return none
    if family.consumeMData == outer.consumeMData then return none
    return some (← synthInstance? (mkApp (mkConst ``CarriersNonempty) outer))
  let mut rebuiltArgs := (args.extract 0 position).push outer
  let mut typed := position + 1
  for arg in args.extract (position + 1) args.size do
    if spelledType arg then
      let kind := if isNominalType env arg then ``NTy
        else arg.consumeMData.getAppFn.constName!.getPrefix
      let substituted := if kind == ``NTy then substituteNTy row arg (some env)
        else if kind == ``NRow then substituteNRow row arg (some env)
        else substituteNRows row arg (some env)
      let some substituted := substituted | return none
      rebuiltArgs := rebuiltArgs.push substituted
      typed := rebuiltArgs.size
    else if let some index := respellIndex? env row arg then
      rebuiltArgs := rebuiltArgs.push index
      typed := rebuiltArgs.size
    else if let some nonempty? ← dependentAtOuter? arg then
      let some nonempty := nonempty? | return none
      rebuiltArgs := rebuiltArgs.push nonempty
      typed := rebuiltArgs.size
    else rebuiltArgs := rebuiltArgs.push arg
  return some (mkAppN t.getAppFn rebuiltArgs, typed)

/-- `respelledOperation?` where the two spellings convert. -/
def respellOperation? (t : Lean.Expr)
    (converts : Lean.Expr → Lean.Expr → MetaM Bool := fun a b => isDefEq a b) :
    MetaM (Option Lean.Expr) := do
  let some (respelled, typed) ← respelledOperation? t | return none
  let operation := mkAppN t.getAppFn (t.getAppArgs.extract 0 typed)
  if operation.hasLooseBVars then return none
  unless ← converts operation (mkAppN respelled.getAppFn (respelled.getAppArgs.extract 0 typed)) do
    return none
  return some respelled

/-- A term with the types and defaults it spells at an induced family
spelled at the outer family, as the caller states them: carriers, rows,
variants, result shapes, and defaults (`NTy.inhabitantOf`), in types and
instances too, which simp leaves as they are. At spelled types the two
spellings are definitionally equal: the induced family's parameter is the
outer family's substituted type. -/
def respellInducedNode (e : Lean.Expr)
    (converts : Lean.Expr → Lean.Expr → MetaM Bool := fun a b => isDefEq a b) :
    MetaM TransformStep := do
  let env ← getEnv
  let .const name _ := e.getAppFn | return .continue
  let args := e.getAppArgs
  -- A substitution by a spelled row, evaluated; a parameter's value read in
  -- the outer view, the value.
  let spelled (row : Lean.Expr) : MetaM (Option Lean.Expr) := do
    let row := row.consumeMData
    if row.isAppOfArity ``Subtype.val 3 then
      return substitutionRow? (← whnfR (row.getArg! 2))
    return if spelledNRow row (some env) then some row else none
  match name, args with
  | ``NTy.subst, #[row, τ] =>
      if let some row ← spelled row then
        if let some τ' := substituteNTy row τ (some env) then return .done τ'
      return .continue
  | ``NRow.subst, #[row, fields] =>
      if let some row ← spelled row then
        if let some fields' := substituteNRow row fields (some env) then return .done fields'
      return .continue
  | ``NRows.subst, #[row, rows] =>
      if let some row ← spelled row then
        if let some rows' := substituteNRows row rows (some env) then return .done rows'
      return .continue
  | ``NTy.ofSkolem, #[_, _, _, τ, value] =>
      if τ.consumeMData.isAppOfArity ``NTy.param 1 then return .done value
      return .continue
  | _, _ => pure ()
  let some family := args[0]? | return .continue
  -- The plain layer names its family at the lent layer of a `Carriers`
  -- (`Carriers.toPlain`), so the family to respell is that one's.
  let carriers := if family.isAppOfArity ``Carriers.toPlain 1 then family.appArg! else family
  let some (θ, outer) := inducedCarriers? carriers | return .continue
  let some row := substitutionRow? θ | return .continue
  let plain := mkApp (mkConst ``Carriers.toPlain) outer
  let carrier (τ : Lean.Expr) : Option Lean.Expr :=
    (substituteNTy row τ (some env)).map (mkApp2 (mkConst ``NTy.carrier) outer)
  let respelled : Option Lean.Expr ← match name, args with
    | ``NTy.carrier, #[_, τ] => pure (carrier τ)
    | ``HList, #[_, fields] =>
        pure ((substituteNRow row fields (some env)).map (mkApp2 (mkConst ``HList) outer))
    -- Environments add only Option around each carrier in the row, so
    -- the same structural substitution guarantees their conversion too.
    | ``HEnv, #[_, fields] =>
        pure ((substituteNRow row fields (some env)).map (mkApp2 (mkConst ``HEnv) outer))
    | ``variantCarrier, #[_, names, rows] =>
        pure ((substituteNRows row rows (some env)).map (mkApp3 (mkConst ``variantCarrier) outer names))
    -- The plain rows and variants a certified struct or enum carries
    -- (`NTy.carrier`): a field row and a variant's payload are spelled at
    -- the induced family too, in the one canonical spelling of the plain
    -- layer (`PlainSpelling`) whichever spelling the term carried.
    | ``NTy.plain, #[_, τ] =>
        pure ((substituteNTy row τ (some env)).map (mkApp2 (mkConst ``NTy.plainOf) plain))
    | ``NTy.plainOf, #[_, τ] =>
        pure ((substituteNTy row τ (some env)).map (mkApp2 (mkConst ``NTy.plainOf) plain))
    | ``HListPlain, #[_, fields] =>
        pure ((substituteNRow row fields (some env)).map (mkApp2 (mkConst ``HListPlainOf) plain))
    | ``HListPlainOf, #[_, fields] =>
        pure ((substituteNRow row fields (some env)).map (mkApp2 (mkConst ``HListPlainOf) plain))
    | ``variantPlain, #[_, names, rows] =>
        pure ((substituteNRows row rows (some env)).map (mkApp3 (mkConst ``variantPlainOf) plain names))
    | ``variantPlainOf, #[_, names, rows] =>
        pure ((substituteNRows row rows (some env)).map (mkApp3 (mkConst ``variantPlainOf) plain names))
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
        let some substituted := substituteNTy row τ (some env) | pure none
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
    | ``NTy.inhabitantOf, #[_, _, τ, _] => do
        let some substituted := substituteNTy row τ (some env) | pure none
        let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
          (← mkEq (mkApp (mkConst ``NTy.inhabitable) substituted) (mkConst ``Bool.true))
        -- The outer family's inhabitants, which the induced family's are
        -- derived from (`CarriersNonempty.instantiate`): the term being
        -- respelled could not exist without them.
        let some nonempty ← synthInstance? (mkApp (mkConst ``CarriersNonempty) outer)
          | pure none
        pure (some (mkApp4 (mkConst ``NTy.inhabitantOf) outer nonempty substituted decided))
    | _, _ => pure none
  -- A respelling of a VALUE the family decides is not a conversion: the
  -- two defaults of a type, and the two orders of a parameter, are one
  -- value carried across the families (`inhabitantOf_instantiate_heq`),
  -- which the carrying's own identity proves and no transparency
  -- unfolds.  The respelled types below are conversions — an induced
  -- family's parameter IS the outer family's substituted type — so only
  -- the values are checked, and a value whose respelling does not
  -- convert keeps the spelling it has, for a rule that carries a proof
  -- to reach (`inhabitantOuterFamily`).
  let decided := [``NTy.inhabitantOf, ``NTy.readOrder].contains name
  match respelled with
  | some value =>
      if decided then
        unless ← isDefEq e value do return .continue
      return .done value
  | none =>
  -- Any other native operation at the induced family, such as a field's
  -- selection, at the outer family.
  match ← respellOperation? e converts with
  | some value => return .done value
  | none => return .continue

/-- Traverse every annotation as well as values with the shared native representation policy. -/
def respellInducedFamily (e : Lean.Expr) : MetaM Lean.Expr :=
  Meta.transform e (post := fun term => respellInducedNode term)

end VerexIR.Proofs.Denote.FamilyTransport
