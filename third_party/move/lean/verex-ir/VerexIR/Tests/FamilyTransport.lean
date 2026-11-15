-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
open VerexIR.Proofs.Denote
set_option quotPrecheck false
local notation "arguments" => (⟨.cons (.int 64 false) .nil, rfl⟩ : TypeArgs)
local notation "calleeType" => (NTy.enum (⟨⟨0⟩, 0⟩ : VerexIR.StructHandle) .nil ["End", "Value"]
  (.cons .nil (.cons (.cons (.param 0) .nil) .nil)) (by decide))
local notation "callerType" => (NTy.enum (⟨⟨0⟩, 0⟩ : VerexIR.StructHandle) .nil ["End", "Value"]
  (.cons .nil (.cons (.cons (.int 64 false) .nil) .nil)) (by decide))
-- A callee's value at its instantiated family is the caller's value at the
-- substituted type, spelled as the caller spells it.
example [family : Carriers] [CarriersNonempty family]
    (value : @NTy.carrier (Carriers.instantiate arguments family) calleeType)
    (other : @NTy.carrier family callerType) (same : value = other) :
    @Eq (@NTy.carrier family callerType) value other := by
  fail_if_success with_reducible exact same
  verex_canonical_families
  with_reducible exact same

-- The conversion policy is fixed even when the caller restricts transparency.
example [family : Carriers] [CarriersNonempty family]
    (value : @NTy.carrier (Carriers.instantiate arguments family) calleeType)
    (other : @NTy.carrier family callerType) (same : value = other) :
    @Eq (@NTy.carrier family callerType) value other := by
  fail_if_success with_reducible exact same
  with_reducible verex_canonical_families
  with_reducible exact same

-- Rewriting under a binder preserves its scope.
example [family : Carriers] [CarriersNonempty family]
    (value : @NTy.carrier (Carriers.instantiate arguments family) calleeType)
    (equal : ∀ other : @NTy.carrier (Carriers.instantiate arguments family) calleeType,
      other = value)
    (other : @NTy.carrier family callerType) :
    @Eq (@NTy.carrier family callerType) other value := by
  fail_if_success with_reducible exact equal other
  verex_canonical_families
  with_reducible exact equal other

example [family : Carriers] [CarriersNonempty family]
    (value : @NTy.carrier (Carriers.instantiate arguments family) (.ghost calleeType))
    (other : @NTy.carrier family (.ghost callerType)) (same : value = other) :
    @Eq (@NTy.carrier family (.ghost callerType)) value other := by
  fail_if_success with_reducible exact same
  verex_canonical_families
  with_reducible exact same

-- Nested variants and collections are spelled at the caller's types too.
example [family : Carriers] [CarriersNonempty family]
    (values : @NTy.carrier (Carriers.instantiate arguments family) (.vector calleeType))
    (others : @NTy.carrier family (.vector callerType)) (same : values = others) :
    @Eq (@NTy.carrier family (.vector callerType)) values others := by
  fail_if_success with_reducible exact same
  verex_canonical_families
  with_reducible exact same

example [family : Carriers] [CarriersNonempty family]
    (value : @NTy.carrier (Carriers.instantiate arguments family)
      (.tuple (.cons calleeType (.cons (.ghost calleeType) .nil))))
    (other : @NTy.carrier family (.tuple (.cons callerType (.cons (.ghost callerType) .nil))))
    (same : value = other) :
    @Eq (@NTy.carrier family (.tuple (.cons callerType (.cons (.ghost callerType) .nil))))
      value other := by
  fail_if_success with_reducible exact same
  verex_canonical_families
  with_reducible exact same

-- A successful search through a caller's vector supplies the existential
-- witness for the same vector returned through a generic callee. Preparing
-- the search must also remove the element transport and rebuilt certificate.
section
local notation "addressArguments" => (⟨.cons .address .nil, rfl⟩ : TypeArgs)

example [family : Carriers] [CarriersNonempty family] (original : VerexIR.SpecVector String)
    (returned : @NTy.carrier (Carriers.instantiate addressArguments family)
      (.vector (.param 0))) (needle : String)
    (linked : returned = @NTy.toSkolem family inferInstance addressArguments (.vector (.param 0)) original)
    (found : (findIndex? (@NTy.eqb family .address) original.values needle
      original.values.size 0).isSome = true) :
    ∃ i : Int, 0 ≤ i ∧ i < returned.values.size ∧ returned.values[i.toNat]? = some needle := by
  verex_leaf
end

-- Destructuring targets substitute their context through Option's explicit
-- element type as well as through the contained Var. A bool slot still changes
-- its context when a neighboring type parameter is instantiated.
open Lean Meta Elab Command in
run_meta do
  let env ← getEnv
  let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
  let address := mkConst ``NTy.address
  let boolean := mkConst ``NTy.bool
  let nil := mkConst ``NRow.nil
  let cons := mkApp2 (mkConst ``NRow.cons)
  let row := cons address nil
  let build (τ : Expr) (present : Bool) : Expr := Id.run do
    let tail := cons boolean nil
    let context := cons τ tail
    let slot := mkAppN (mkConst ``Var.there)
      #[tail, τ, boolean, mkApp2 (mkConst ``Var.here) nil boolean]
    let targetType := mkApp2 (mkConst ``Var) context boolean
    let target := if present then mkApp2 (mkConst ``Option.some [0]) targetType slot
      else mkApp (mkConst ``Option.none [0]) targetType
    let discardedType := mkApp2 (mkConst ``Var) context τ
    let rest := mkAppN (mkConst ``Vars.cons) #[context, τ, nil,
      mkApp (mkConst ``Option.none [0]) discardedType, mkApp (mkConst ``Vars.nil) context]
    return mkAppN (mkConst ``Vars.cons) #[context, boolean, cons τ nil, target, rest]
  for present in [true, false] do
    let some rebuilt := FamilyTransport.respellIndex? env row (build parameter present)
      | throwError "destructuring targets were not respelled"
    let expected := build address present
    unless rebuilt == expected do
      throwError "optional target retained its generic context, type, or variable"
    -- Kernel-check the entire rebuilt constructor chain at its concrete type.
    addDecl (.defnDecl {
      name := if present then `VerexIR.Tests.FamilyTransport.someTarget
        else `VerexIR.Tests.FamilyTransport.noneTarget
      levelParams := [], type := (← inferType expected), value := rebuilt,
      hints := .abbrev, safety := .safe })

  let generic := build parameter true
  let context := generic.getArg! 0
  let someTarget := generic.getArg! 3
  let targetType := someTarget.getArg! 0
  let checkOpaque (name : Name) (type : Expr) (replace : Expr → Expr) :=
    withLocalDeclD name type fun opaqueValue => do
      unless (FamilyTransport.respellIndex? env row (replace opaqueValue)).isNone do
        throwError "opaque dependent index field was partially substituted"
  checkOpaque `slot targetType fun slot =>
    mkAppN generic.getAppFn (generic.getAppArgs.set! 3
      (mkApp2 (mkConst ``Option.some [0]) targetType slot))
  checkOpaque `target (mkApp (mkConst ``Option [0]) targetType) fun target =>
    mkAppN generic.getAppFn (generic.getAppArgs.set! 3 target)
  checkOpaque `rest (mkApp2 (mkConst ``Vars) context (cons parameter nil)) fun rest =>
    mkAppN generic.getAppFn (generic.getAppArgs.set! 4 rest)

  let typeArgs ← whnf (mkConst ``TypeArgs)
  let substitution := mkApp4 (mkConst ``Subtype.mk [0]) (mkConst ``NRow)
    (typeArgs.getArg! 1) row (← mkEqRefl (mkConst ``Bool.true))
  withLocalDeclD `outer (mkConst ``Carriers) fun outer => do
    withLocalDeclD `nonempty (mkApp (mkConst ``CarriersNonempty) outer) fun nonempty => do
      let family := mkApp3 (mkConst ``Carriers.instantiate) substitution outer nonempty
      let resultRow := cons boolean (cons parameter nil)
      let rejectOperation (index : Expr) : MetaM Unit := do
        let operation := mkAppN (mkConst ``Vars.set) #[family, context, resultRow, index]
        unless (← FamilyTransport.respelledOperation? operation).isNone do
          throwError "operation retained an old-context index beside substituted rows"
      withLocalDeclD `target (mkApp (mkConst ``Option [0]) targetType) fun target => do
        rejectOperation (mkAppN generic.getAppFn (generic.getAppArgs.set! 3 target))
      withLocalDeclD `index (mkApp2 (mkConst ``Vars) context resultRow) rejectOperation
