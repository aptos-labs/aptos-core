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

section PlainStorageInterfaceTests
open VerexIR VerexIR.Validation

variable {unit : ValidatedUnit} [FunctionCarriers unit]

local notation "resA" => (NTy.struct (⟨⟨0⟩, 0⟩ : StructHandle) .nil (.cons (.int 64 false) .nil))
local notation "resB" => (NTy.struct (⟨⟨0⟩, 1⟩ : StructHandle) .nil (.cons .bool .nil))
local notation "genRes" =>
  (NTy.struct (⟨⟨0⟩, 2⟩ : StructHandle) (.cons (.param 0) .nil) (.cons (.param 0) .nil))
local notation "instRes" =>
  (NTy.struct (⟨⟨0⟩, 2⟩ : StructHandle) (.cons (.int 64 false) .nil) (.cons (.int 64 false) .nil))

-- Fast path 1: distinct resources (premise-free, no `NTy.Holds` needed on `bad`)
example [Θ : Skolems unit] (m : Memory unit) (k1 k2 : StorageKey)
    (bad : @NTy.plain Θ.toCarriers resA) :
    Skolems.plainAt resB .nil k2 (Skolems.writePlain resA .nil k1 (some bad) m) =
      Skolems.plainAt resB .nil k2 m := by
  simp only [vir_denote_norm]

-- Fast path 1 negative check: same declaration differing ONLY by unresolved type parameters MUST decline!
example [Θ : Skolems unit] (m : Memory unit) (k : StorageKey)
    (v : @NTy.plain Θ.toCarriers genRes) :
    Skolems.plainAt genRes (.cons (.param 1) .nil) k
      (Skolems.writePlain genRes (.cons (.param 0) .nil) k (some v) m) =
    Skolems.plainAt genRes (.cons (.param 1) .nil) k
      (Skolems.writePlain genRes (.cons (.param 0) .nil) k (some v) m) := by
  fail_if_success
    have : Skolems.plainAt genRes (.cons (.param 1) .nil) k
        (Skolems.writePlain genRes (.cons (.param 0) .nil) k (some v) m) =
      Skolems.plainAt genRes (.cons (.param 1) .nil) k m := by
      simp only [reducePlainAtWritePlain]
  rfl

-- Fast path 2: `none` (premise-free)
example [Θ : Skolems unit] (m : Memory unit) (k : StorageKey) :
    Skolems.plainAt resA .nil k (Skolems.writePlain resA .nil k none m) = none := by
  simp only [vir_denote_norm]

-- Fast path 3: `otherKey ≠ key` (premise-free, no `NTy.Holds` needed on `bad`)
example [Θ : Skolems unit] (m : Memory unit) (a1 a2 : String) (hne : a2 ≠ a1)
    (bad : @NTy.plain Θ.toCarriers resA) :
    Skolems.plainAt resA .nil (.address a2)
      (Skolems.writePlain resA .nil (.address a1) (some bad) m) =
      Skolems.plainAt resA .nil (.address a2) m := by
  simp only [vir_denote_norm]

-- Fast path 4a + `NTy.ofPlain?_forget`: normal form ends in `c`, NOT `ofDeepHolds (forget c) _`
example [Θ : Skolems unit] (m : Memory unit) (k : StorageKey)
    (c : @NTy.carrier Θ.toCarriers resA) :
    (Skolems.plainAt resA .nil k
      (Skolems.writePlain resA .nil k (some (NTy.forget resA c)) m)).bind
      (NTy.ofPlain? resA) = some c := by
  simp only [vir_denote_norm]

-- Fast path 4c: `LCtx` `NTy.Holds`
example [Θ : Skolems unit] (m : Memory unit) (k : StorageKey)
    (v : @NTy.plain Θ.toCarriers resA) (hv : @NTy.Holds Θ.toCarriers resA v) :
    Skolems.plainAt resA .nil k (Skolems.writePlain resA .nil k (some v) m) = some v := by
  simp only [vir_denote_norm]

-- Outer-frame instantiation laws (`plainAtOuterFrame`, `writePlainOuterFrame`, `optionPlainTransportIdentity`)
example (outer : Skolems unit) (m : Memory unit) (k : StorageKey) :
    @Skolems.plainAt unit _ (Skolems.instantiate arguments outer) genRes (.cons (.param 0) .nil) k m =
      @Skolems.plainAt unit _ outer instRes (.cons (.int 64 false) .nil) k m := by
  simp only [vir_denote_norm]

example (outer : Skolems unit) (m : Memory unit) (k : StorageKey)
    (v : Option (@NTy.plain (Skolems.instantiate arguments outer).toCarriers genRes)) :
    @Skolems.writePlain unit _ (Skolems.instantiate arguments outer) genRes
      (.cons (.param 0) .nil) k v m =
      @Skolems.writePlain unit _ outer instRes (.cons (.int 64 false) .nil) k v m := by
  simp only [vir_denote_norm]

-- `TableMemory.allocationResource` preservation under `Skolems.writePlain`
example [Θ : Skolems unit] (m : Memory unit) (k1 k2 : StorageKey)
    (v : Option (@NTy.plain Θ.toCarriers resA)) :
    Skolems.writePlain resA .nil k1 v m TableMemory.allocationResource k2 =
      m TableMemory.allocationResource k2 := by
  simp only [vir_denote_norm]

-- `verex_frame_writes` (`closeFrameWrites`) over `Skolems.writePlain`
example [Θ : Skolems unit] (m : Memory unit) (k : StorageKey)
    (v : Option (@NTy.plain Θ.toCarriers resA)) :
    (∀ k' : StorageKey, k' ≠ k →
      Skolems.writePlain resA .nil k v m (Skolems.resource resA .nil) k' =
        m (Skolems.resource resA .nil) k') ∧
    (∀ r : ResourceType, r ≠ Skolems.resource resA .nil →
      Skolems.writePlain resA .nil k v m r = m r) := by
  verex_frame_writes

end PlainStorageInterfaceTests
